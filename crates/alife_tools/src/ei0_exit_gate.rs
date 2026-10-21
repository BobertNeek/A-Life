//! Reproducible Era 0 lifecycle and promotion-gate evidence.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};

use alife_core::{
    ActiveChallengeResult, Blake3Digest, CreatureGenome, FoundationWeightAsset,
    GeneticLineageProvenance, GenomeId, LineageId, OrganismId, PhenotypeHash, PolicyBackend,
    ScaffoldContractError, SensorProfile, Tick, ACTIVE_CHALLENGE_COUNT,
};
use alife_game_app::{PopulationStabilityReceipt, MINIMUM_POST_RESTORE_TICKS};
use alife_gpu_backend::closed_loop_shader_bundle_digest;
use alife_training::expected_n2048_creature_phenotype_hash;
use alife_world::{
    HabitatActor, HabitatBreedingKind, HabitatBreedingReceipt, HabitatId, HabitatMode,
    HeadlessWorldSignatureDigest, HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION,
};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::RawValue;

pub const EI0_EXIT_GATE_SCHEMA_VERSION: u16 = 3;
const EI0_SCHEMA_3_HISTORICAL_PRODUCER_COMMIT: &str = "ab806c2c6e4910a29927cea4746f4007d75f3f15";
const EI0_SCHEMA_3_HISTORICAL_VERSION: u16 = 3;
const COMMITTED_EI0_REPORT_PATH: &str = "crates/alife_tools/reports/ei0_exit_gate_report.json";
const REQUIRED_GPU_ADAPTER: &str = "NVIDIA GeForce RTX 3050";
const REQUIRED_GPU_BACKEND_API: &str = "vulkan";
const SOURCE_CONTRACT_PATHS: &[&str] = &[
    "Cargo.lock",
    "Cargo.toml",
    "crates/alife_world/src/headless.rs",
    "crates/alife_game_app/src/habitat_lab_commands.rs",
    "crates/alife_game_app/src/composite_population_runtime.rs",
    "crates/alife_game_app/src/production_conversation_lineage_ui.rs",
    "crates/alife_training/src/active_battery.rs",
    "crates/alife_tools/src/ei0_exit_gate.rs",
    "crates/alife_tools/src/bin/ei0_exit_gate.rs",
];

#[derive(Debug, thiserror::Error)]
pub enum Ei0ExitGateError {
    #[error("core contract failed: {0}")]
    Core(#[from] ScaffoldContractError),
    #[error("GPU active battery failed: {0}")]
    Training(#[from] alife_training::TrainingError),
    #[error("gate report I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("gate report JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("gate evidence is inconsistent: {0}")]
    Evidence(&'static str),
    #[error("gate source binding failed: {0}")]
    Source(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0LifetimeEvidence {
    pub memory_records: u32,
    pub lifetime_weights: u32,
    pub state_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0BirthReceipt {
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub lineage_id: LineageId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub generation: u32,
    pub conception_seed: u64,
    pub ordinary_birth: bool,
    pub provenance: GeneticLineageProvenance,
    pub foundation_id: u64,
    pub foundation_version: u16,
    pub compatibility_family_id: u64,
    pub breeding_kind: HabitatBreedingKind,
    pub actor: HabitatActor,
    pub breeding_receipt: HabitatBreedingReceipt,
    pub cognition_policy: PolicyBackend,
    pub child_phenotype_hash: PhenotypeHash,
    pub post_restore_ticks: u32,
    pub first_parent_lifetime: Ei0LifetimeEvidence,
    pub second_parent_lifetime: Ei0LifetimeEvidence,
    pub child_lifetime: Ei0LifetimeEvidence,
    pub gpu_intent_sequence_id: Option<u64>,
    pub gpu_intent_world_tick: Option<Tick>,
    pub gpu_selected_mate: Option<OrganismId>,
    pub gpu_pre_action_world_digest: Option<HeadlessWorldSignatureDigest>,
    pub gpu_same_seed_wrong_world_rejected: Option<bool>,
    pub gpu_later_world_rejected: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0LaneReceipt {
    pub mode: HabitatMode,
    pub habitat_id: HabitatId,
    pub births: Vec<Ei0BirthReceipt>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0EvidenceDigests {
    pub source_genomes: BTreeMap<String, String>,
    pub foundation_weights: Option<String>,
    pub shader_bundle: Option<String>,
    pub portable_save: Option<String>,
    pub archive_manifests: BTreeMap<String, String>,
    pub archive_composite_assets: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0ResidentIdentityReceipt {
    pub organism_id: OrganismId,
    pub genome_id: GenomeId,
    pub generation: u32,
    pub phenotype_hash: PhenotypeHash,
    pub restored_from_save: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0LifecycleGateReport {
    pub schema_version: u16,
    pub founder_count: usize,
    pub live_population_count: usize,
    pub generation_count: u32,
    pub run_observed: bool,
    pub portable_save_round_trip: bool,
    pub tampered_save_rejected: bool,
    pub tampered_provenance_rejected: bool,
    pub restored_population_count: usize,
    pub post_restore_ticks: u32,
    pub stability: PopulationStabilityReceipt,
    pub same_seed_wrong_world_rejected: bool,
    pub later_world_rejected: bool,
    pub archive_birth_manifest_count: u64,
    pub lineage_compare_passed: bool,
    pub no_lifetime_state_inherited: bool,
    pub player_directed_wild_breeding_rejected: bool,
    pub creature_directed_managed_breeding_rejected: bool,
    pub lanes: Vec<Ei0LaneReceipt>,
    pub population_genomes: Vec<Ei0GenomeEvidence>,
    pub population_residents: Vec<Ei0ResidentIdentityReceipt>,
    pub evidence_digests: Ei0EvidenceDigests,
}

/// A genome embedded in an EI0 evidence report.
///
/// Current reports retain the fully typed runtime genome. Historical v1
/// reports retain their exact JSON object instead: those genomes predate the
/// required biochemical graph and must not be promoted into current runtime
/// state by inventing one.
#[derive(Debug, Clone)]
// Keep the public Current(CreatureGenome) construction and pattern-match API;
// historical evidence owns its exact raw JSON separately.
#[allow(clippy::large_enum_variant)]
pub enum Ei0GenomeEvidence {
    Current(CreatureGenome),
    HistoricalV1(Box<RawValue>),
}

impl Ei0GenomeEvidence {
    pub const fn is_historical_v1(&self) -> bool {
        matches!(self, Self::HistoricalV1(_))
    }

    fn current(&self) -> Option<&CreatureGenome> {
        match self {
            Self::Current(genome) => Some(genome),
            Self::HistoricalV1(_) => None,
        }
    }

    fn identity(&self) -> Result<Ei0GenomeIdentity, Ei0ExitGateError> {
        match self {
            Self::Current(genome) => Ok(Ei0GenomeIdentity {
                id: genome.id,
                parent_genome_ids: genome.parent_genome_ids.clone(),
                lineage_id: genome.lineage_id,
                conception_seed: genome.conception_seed,
            }),
            Self::HistoricalV1(raw) => historical_genome_identity(raw.get()),
        }
    }

    fn digest(&self) -> Result<String, serde_json::Error> {
        match self {
            Self::Current(genome) => serde_json::to_vec(genome).map(|bytes| digest_bytes(&bytes)),
            Self::HistoricalV1(raw) => Ok(digest_bytes(&compact_raw_json(raw.get()))),
        }
    }
}

impl PartialEq for Ei0GenomeEvidence {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Current(left), Self::Current(right)) => left == right,
            (Self::HistoricalV1(left), Self::HistoricalV1(right)) => left.get() == right.get(),
            _ => false,
        }
    }
}

impl Serialize for Ei0GenomeEvidence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Current(genome) => genome.serialize(serializer),
            Self::HistoricalV1(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Ei0GenomeEvidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        if let Ok(genome) = serde_json::from_str::<CreatureGenome>(raw.get()) {
            return Ok(Self::Current(genome));
        }
        let value: serde_json::Value = serde_json::from_str(raw.get()).map_err(D::Error::custom)?;
        if value
            .get("schema_version")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
        {
            return Err(D::Error::custom(
                "EI0 genome is neither a current genome nor historical schema v1 evidence",
            ));
        }
        historical_genome_identity_from_value(&value).map_err(D::Error::custom)?;
        Ok(Self::HistoricalV1(raw))
    }
}

impl From<CreatureGenome> for Ei0GenomeEvidence {
    fn from(genome: CreatureGenome) -> Self {
        Self::Current(genome)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Ei0GenomeIdentity {
    id: GenomeId,
    parent_genome_ids: Vec<GenomeId>,
    lineage_id: LineageId,
    conception_seed: u64,
}

fn historical_genome_identity(raw: &str) -> Result<Ei0GenomeIdentity, Ei0ExitGateError> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    historical_genome_identity_from_value(&value).map_err(Ei0ExitGateError::Source)
}

fn historical_genome_identity_from_value(
    value: &serde_json::Value,
) -> Result<Ei0GenomeIdentity, String> {
    let required_u64 = |field: &'static str| {
        value
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("historical EI0 genome is missing {field}"))
    };
    let parent_genome_ids = value
        .get("parent_genome_ids")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "historical EI0 genome is missing parent_genome_ids".to_string())?
        .iter()
        .map(|parent| {
            parent
                .as_u64()
                .map(GenomeId)
                .ok_or_else(|| "historical EI0 parent genome ID is invalid".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Ei0GenomeIdentity {
        id: GenomeId(required_u64("id")?),
        parent_genome_ids,
        lineage_id: LineageId(required_u64("lineage_id")?),
        conception_seed: required_u64("conception_seed")?,
    })
}

fn compact_raw_json(raw: &str) -> Vec<u8> {
    let mut compact = Vec::with_capacity(raw.len());
    let mut in_string = false;
    let mut escaped = false;
    for byte in raw.bytes() {
        if in_string {
            compact.push(byte);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
            compact.push(byte);
        } else if !byte.is_ascii_whitespace() {
            compact.push(byte);
        }
    }
    compact
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0GpuBatteryReceipt {
    pub organism_id: OrganismId,
    pub source_creature_genome_id: GenomeId,
    pub brain_genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: LineageId,
    pub phenotype_hash: PhenotypeHash,
    pub foundation_id: u64,
    pub foundation_version: u32,
    pub compatibility_family_id: u64,
    pub policy_backend: PolicyBackend,
    pub completed_challenges: usize,
    pub challenge_results: Vec<ActiveChallengeResult>,
    pub challenge_worlds: u32,
    pub gpu_dispatches: u64,
    pub sealed_outcomes: u64,
    pub sleep_consolidations: u32,
    pub slm_enabled: bool,
    pub adapter_name: String,
    pub backend_api: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0HeuristicBaselineBoundary {
    pub source_backend: String,
    pub promotion_eligible: bool,
    pub hidden_promotion_trials: u64,
    pub unknown_measures: Vec<String>,
    pub unknown_measures_preserved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ei0EvidenceStatus {
    Pass,
    Fail,
    Unknown,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0ClauseEvidence {
    pub status: Ei0EvidenceStatus,
    pub detail: String,
}

impl Ei0ClauseEvidence {
    pub const fn passed(&self) -> bool {
        matches!(self.status, Ei0EvidenceStatus::Pass)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0ExitClauses {
    pub run: Ei0ClauseEvidence,
    pub observe: Ei0ClauseEvidence,
    pub save_load: Ei0ClauseEvidence,
    pub wild_breed: Ei0ClauseEvidence,
    pub managed_breed: Ei0ClauseEvidence,
    pub test: Ei0ClauseEvidence,
    pub archive: Ei0ClauseEvidence,
    pub compare: Ei0ClauseEvidence,
    pub stable_multi_generation_population: Ei0ClauseEvidence,
    pub gpu_policy_identity: Ei0ClauseEvidence,
    pub no_hidden_policy_control: Ei0ClauseEvidence,
}

impl Ei0ExitClauses {
    pub fn all_passed(&self) -> bool {
        self.iter().all(Ei0ClauseEvidence::passed)
    }

    fn iter(&self) -> impl Iterator<Item = &Ei0ClauseEvidence> {
        [
            &self.run,
            &self.observe,
            &self.save_load,
            &self.wild_breed,
            &self.managed_breed,
            &self.test,
            &self.archive,
            &self.compare,
            &self.stable_multi_generation_population,
            &self.gpu_policy_identity,
            &self.no_hidden_policy_control,
        ]
        .into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0ExitVerdict {
    pub era0_exit_gate_passed: bool,
    pub era1_promotion_evaluated: bool,
    pub era1_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0ArtifactBinding {
    pub producing_source_commit: String,
    pub producing_source_tree: String,
    pub source_contract_paths: Vec<String>,
    pub source_contract_digest: String,
    pub adapter_name: String,
    pub backend_api: String,
    pub causal_birth_receipts_digest: String,
    pub gpu_receipts_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0ExitGateReport {
    pub schema_version: u16,
    pub verdict: Ei0ExitVerdict,
    pub clauses: Ei0ExitClauses,
    pub lifecycle: Option<Ei0LifecycleGateReport>,
    pub gpu_tests: Vec<Ei0GpuBatteryReceipt>,
    pub heuristic_baseline: Option<Ei0HeuristicBaselineBoundary>,
    pub evidence_digests: Ei0EvidenceDigests,
    pub artifact_binding: Option<Ei0ArtifactBinding>,
    pub operational_error: Option<String>,
}

fn clause(passed: bool, detail: impl Into<String>) -> Ei0ClauseEvidence {
    Ei0ClauseEvidence {
        status: if passed {
            Ei0EvidenceStatus::Pass
        } else {
            Ei0EvidenceStatus::Fail
        },
        detail: detail.into(),
    }
}

fn wild_birth_semantics_are_exact(
    birth: &Ei0BirthReceipt,
    expected_world_signature_schema: Option<u16>,
) -> bool {
    let breeding = &birth.breeding_receipt;
    birth.breeding_kind == HabitatBreedingKind::CreatureChosen
        && breeding.kind == birth.breeding_kind
        && birth.actor == HabitatActor::Organism(breeding.first_parent)
        && breeding.actor == birth.actor
        && birth.cognition_policy == PolicyBackend::NeuralClosedLoopGpu
        && breeding.cognition_policy == birth.cognition_policy
        && birth
            .gpu_intent_sequence_id
            .is_some_and(|sequence| sequence > 0)
        && birth.gpu_intent_world_tick == Some(breeding.tick)
        && birth.gpu_selected_mate == Some(breeding.second_parent)
        && birth.gpu_pre_action_world_digest.is_some_and(|digest| {
            expected_world_signature_schema.is_none_or(|schema| digest.schema_version == schema)
                && digest.words.iter().any(|word| *word != 0)
        })
        && birth.gpu_same_seed_wrong_world_rejected == Some(true)
        && birth.gpu_later_world_rejected == Some(true)
}

fn evaluate_clauses(
    lifecycle: &Ei0LifecycleGateReport,
    gpu_tests: &[Ei0GpuBatteryReceipt],
    baseline: &Ei0HeuristicBaselineBoundary,
) -> Ei0ExitClauses {
    let wild_lane = lifecycle
        .lanes
        .iter()
        .find(|lane| lane.mode == HabitatMode::Wild);
    let managed_lane = lifecycle
        .lanes
        .iter()
        .find(|lane| lane.mode == HabitatMode::Managed);
    let wild_breed = lifecycle.player_directed_wild_breeding_rejected
        && wild_lane.is_some_and(|lane| {
            let sequences = lane
                .births
                .iter()
                .filter_map(|birth| {
                    birth
                        .gpu_intent_sequence_id
                        .map(|sequence| (birth.breeding_receipt.first_parent.raw(), sequence))
                })
                .collect::<BTreeSet<_>>();
            lane.births.len() == 3
                && sequences.len() == lane.births.len()
                && lane.births.iter().all(|birth| {
                    wild_birth_semantics_are_exact(
                        birth,
                        Some(HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION),
                    )
                })
        });
    let managed_breed = lifecycle.creature_directed_managed_breeding_rejected
        && managed_lane.is_some_and(|lane| {
            lane.births.len() == 3
                && lane.births.iter().all(|birth| {
                    birth.breeding_kind == HabitatBreedingKind::Explicit
                        && birth.actor == HabitatActor::Player
                        && birth.breeding_receipt.kind == birth.breeding_kind
                        && birth.breeding_receipt.actor == birth.actor
                        && birth.cognition_policy == PolicyBackend::NeuralClosedLoopGpu
                        && birth.gpu_intent_sequence_id.is_none()
                })
        });
    let tested = gpu_tests.len() == 2
        && gpu_tests.iter().all(|receipt| {
            receipt.completed_challenges == ACTIVE_CHALLENGE_COUNT
                && receipt.challenge_worlds == ACTIVE_CHALLENGE_COUNT as u32
                && receipt.gpu_dispatches == receipt.sealed_outcomes
                && receipt.gpu_dispatches >= ACTIVE_CHALLENGE_COUNT as u64
                && receipt.sleep_consolidations >= 1
                && !receipt.slm_enabled
                && receipt.adapter_name == REQUIRED_GPU_ADAPTER
                && receipt.backend_api == REQUIRED_GPU_BACKEND_API
        });
    let gpu_policy_identity = tested
        && lifecycle.lanes.iter().zip(gpu_tests).all(|(lane, gpu)| {
            lane.births.last().is_some_and(|birth| {
                gpu.organism_id == birth.organism_id
                    && gpu.source_creature_genome_id == birth.genome_id
                    && gpu.brain_genome_id == birth.genome_id
                    && gpu.parent_genome_ids == birth.parent_genome_ids
                    && gpu.lineage_id == birth.lineage_id
                    && gpu.foundation_id == birth.foundation_id
                    && gpu.foundation_version == u32::from(birth.foundation_version)
                    && gpu.compatibility_family_id == birth.compatibility_family_id
                    && gpu.phenotype_hash == birth.child_phenotype_hash
                    && gpu.policy_backend == PolicyBackend::NeuralClosedLoopGpu
            })
        });
    let no_hidden_policy_control = gpu_policy_identity
        && baseline.source_backend == "HeuristicBaseline"
        && !baseline.promotion_eligible
        && baseline.hidden_promotion_trials == 0
        && baseline.unknown_measures_preserved;
    Ei0ExitClauses {
        run: clause(
            lifecycle.founder_count == 8
                && lifecycle.live_population_count == 14
                && lifecycle.generation_count == 3,
            "8 restored founders and 14 live creatures across generations 0-2",
        ),
        observe: clause(
            lifecycle.run_observed,
            "every restored founder produced a grounded perception frame",
        ),
        save_load: clause(
            lifecycle.portable_save_round_trip
                && lifecycle.tampered_save_rejected
                && lifecycle.restored_population_count == 8
                && lifecycle.post_restore_ticks >= MINIMUM_POST_RESTORE_TICKS,
            "composite genomes, foundation, and lifetime assets restored before 128 ticks",
        ),
        wild_breed: clause(
            wild_breed,
            "Wild births consume GPU-selected Contact/Interact target receipts",
        ),
        managed_breed: clause(
            managed_breed,
            "Managed births enter through the player command with Player authority",
        ),
        test: clause(
            tested,
            "both final genomes completed the 15-case GPU battery",
        ),
        archive: clause(
            lifecycle.archive_birth_manifest_count == 14,
            "all 14 complete composite genomes have immutable birth manifests",
        ),
        compare: clause(
            lifecycle.lineage_compare_passed && lifecycle.tampered_provenance_rejected,
            "archive reloads match full composite provenance and reject tampering",
        ),
        stable_multi_generation_population: clause(
            lifecycle.live_population_count == 14
                && lifecycle.no_lifetime_state_inherited
                && lifecycle.population_genomes.len() == 14
                && lifecycle.stability.elapsed_ticks == MINIMUM_POST_RESTORE_TICKS
                && lifecycle.stability.end_tick.raw()
                    .saturating_sub(lifecycle.stability.start_tick.raw())
                    == u64::from(MINIMUM_POST_RESTORE_TICKS)
                && lifecycle.stability.start_world_digest
                    != lifecycle.stability.end_world_digest
                && lifecycle.stability.start_ecology_metrics
                    != lifecycle.stability.end_ecology_metrics
                && lifecycle.stability.end_ecology_metrics.resources_spawned > 0
                && lifecycle.stability.start_residents == lifecycle.stability.end_residents
                && lifecycle.same_seed_wrong_world_rejected
                && lifecycle.later_world_rejected
                && wild_lane
                    .and_then(|lane| lane.births.first())
                    .and_then(|birth| birth.gpu_pre_action_world_digest)
                    == Some(lifecycle.stability.end_world_digest),
            "128 advance_tick calls evolved ecology, retained residents, and bound the next GPU action",
        ),
        gpu_policy_identity: clause(
            gpu_policy_identity,
            "tested phenotype identity binds final composite genome to GPU execution",
        ),
        no_hidden_policy_control: clause(
            no_hidden_policy_control,
            "HeuristicBaseline remains non-promotional with zero hidden trials and UNKNOWN data",
        ),
    }
}

pub fn validate_committed_ei0_exit_gate_report(
    report: &Ei0ExitGateReport,
) -> Result<(), Ei0ExitGateError> {
    if report.schema_version != EI0_EXIT_GATE_SCHEMA_VERSION
        || !report.verdict.era0_exit_gate_passed
        || report.verdict.era1_promotion_evaluated
        || !report.clauses.all_passed()
        || report.operational_error.is_some()
    {
        return Err(Ei0ExitGateError::Evidence(
            "committed report verdict is not a passing Era 0-only receipt",
        ));
    }
    let lifecycle = report.lifecycle.as_ref().ok_or(Ei0ExitGateError::Evidence(
        "committed report is missing lifecycle evidence",
    ))?;
    let historical_genomes = lifecycle
        .population_genomes
        .iter()
        .all(Ei0GenomeEvidence::is_historical_v1);
    if !historical_genomes
        && lifecycle
            .population_genomes
            .iter()
            .any(Ei0GenomeEvidence::is_historical_v1)
    {
        return Err(Ei0ExitGateError::Evidence(
            "committed report mixes current and historical genome evidence",
        ));
    }
    let baseline = report
        .heuristic_baseline
        .as_ref()
        .ok_or(Ei0ExitGateError::Evidence(
            "committed report is missing the heuristic boundary",
        ))?;
    let binding = report
        .artifact_binding
        .as_ref()
        .ok_or(Ei0ExitGateError::Evidence(
            "committed report is missing its source binding",
        ))?;
    let root = workspace_root();
    let current_head = git_output(&root, &["rev-parse", "HEAD"])?;
    let exact_historical_baseline = is_exact_committed_historical_baseline(report, binding, &root)?;
    if binding.producing_source_commit != current_head && !exact_historical_baseline {
        return Err(Ei0ExitGateError::Evidence(
            "non-current report is not the exact historical baseline",
        ));
    }

    let expected_paths = SOURCE_CONTRACT_PATHS
        .iter()
        .map(|path| (*path).to_string())
        .collect::<Vec<_>>();
    if binding.source_contract_paths != expected_paths {
        return Err(Ei0ExitGateError::Evidence(
            "artifact source-contract path set changed",
        ));
    }
    if source_contract_digest_at_revision(
        &root,
        &binding.producing_source_commit,
        SOURCE_CONTRACT_PATHS,
    )? != binding.source_contract_digest
    {
        return Err(Ei0ExitGateError::Evidence(
            "bound source files do not match the report binding",
        ));
    }
    let bound_tree = git_output(
        &root,
        &[
            "rev-parse",
            &format!("{}^{{tree}}", binding.producing_source_commit),
        ],
    )?;
    if bound_tree != binding.producing_source_tree {
        return Err(Ei0ExitGateError::Evidence(
            "producing source commit does not resolve to the recorded tree",
        ));
    }
    if !historical_genomes {
        let mut source_diff = Command::new("git");
        source_diff
            .current_dir(&root)
            .args(["diff", "--quiet", &binding.producing_source_commit, "--"])
            .args(SOURCE_CONTRACT_PATHS);
        if !source_diff
            .status()
            .map_err(|error| Ei0ExitGateError::Source(error.to_string()))?
            .success()
        {
            return Err(Ei0ExitGateError::Evidence(
                "relevant source differs from the producing commit",
            ));
        }
    }

    let current_assets_match = if historical_genomes {
        true
    } else {
        let current_foundation =
            FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
        report.evidence_digests.foundation_weights.as_deref()
            == Some(format_blake3(current_foundation.digest()).as_str())
            && report.evidence_digests.shader_bundle.as_deref()
                == Some(format_blake3(closed_loop_shader_bundle_digest()).as_str())
    };
    if report.evidence_digests != lifecycle.evidence_digests
        || !current_assets_match
        || report
            .evidence_digests
            .foundation_weights
            .as_deref()
            .is_none_or(|digest| !valid_blake3_text(digest))
        || report
            .evidence_digests
            .shader_bundle
            .as_deref()
            .is_none_or(|digest| !valid_blake3_text(digest))
        || report.evidence_digests.source_genomes.len() != 14
        || report.evidence_digests.archive_manifests.len() != 14
        || report.evidence_digests.archive_composite_assets
            != report.evidence_digests.source_genomes
        || report
            .evidence_digests
            .portable_save
            .as_deref()
            .is_none_or(|digest| !valid_blake3_text(digest))
        || report
            .evidence_digests
            .archive_manifests
            .values()
            .any(|digest| !valid_blake3_text(digest))
    {
        return Err(Ei0ExitGateError::Evidence(
            "committed report asset digests do not recompute",
        ));
    }
    let reported_source_digests = report
        .evidence_digests
        .source_genomes
        .values()
        .cloned()
        .collect::<BTreeSet<_>>();
    let recomputed_source_digests = lifecycle
        .population_genomes
        .iter()
        .map(Ei0GenomeEvidence::digest)
        .collect::<Result<BTreeSet<_>, _>>()?;
    if reported_source_digests != recomputed_source_digests {
        return Err(Ei0ExitGateError::Evidence(
            "population genomes do not match the source digest set",
        ));
    }

    if binding.causal_birth_receipts_digest != digest_bytes(&serde_json::to_vec(&lifecycle.lanes)?)
        || binding.gpu_receipts_digest != digest_bytes(&serde_json::to_vec(&report.gpu_tests)?)
    {
        return Err(Ei0ExitGateError::Evidence(
            "causal or GPU receipt digest changed",
        ));
    }
    let genomes = lifecycle
        .population_genomes
        .iter()
        .map(|genome| genome.identity().map(|identity| (identity.id.0, genome)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let residents = lifecycle
        .population_residents
        .iter()
        .map(|resident| (resident.organism_id.raw(), resident))
        .collect::<BTreeMap<_, _>>();
    if genomes.len() != 14 || residents.len() != 14 {
        return Err(Ei0ExitGateError::Evidence(
            "resident identity receipt is incomplete",
        ));
    }
    for lane in &lifecycle.lanes {
        for birth in &lane.births {
            let genome = genomes
                .get(&birth.genome_id.0)
                .ok_or(Ei0ExitGateError::Evidence("birth receipt genome is absent"))?;
            let resident =
                residents
                    .get(&birth.organism_id.raw())
                    .ok_or(Ei0ExitGateError::Evidence(
                        "birth receipt resident is absent",
                    ))?;
            let breeding = &birth.breeding_receipt;
            let first_parent =
                residents
                    .get(&breeding.first_parent.raw())
                    .ok_or(Ei0ExitGateError::Evidence(
                        "birth receipt first parent is absent",
                    ))?;
            let second_parent =
                residents
                    .get(&breeding.second_parent.raw())
                    .ok_or(Ei0ExitGateError::Evidence(
                        "birth receipt second parent is absent",
                    ))?;
            let phenotype_mismatch = match genome.current() {
                Some(genome) => {
                    expected_n2048_creature_phenotype_hash(genome)? != birth.child_phenotype_hash
                }
                None => false,
            };
            let genome_identity = genome.identity()?;
            if genome_identity.parent_genome_ids != birth.parent_genome_ids
                || birth.parent_genome_ids.as_slice()
                    != [first_parent.genome_id, second_parent.genome_id]
                || genome_identity.lineage_id != birth.lineage_id
                || genome_identity.conception_seed != birth.conception_seed
                || resident.genome_id != birth.genome_id
                || resident.generation != birth.generation
                || resident.phenotype_hash != birth.child_phenotype_hash
                || breeding.habitat_id != lane.habitat_id
                || breeding.kind != birth.breeding_kind
                || breeding.actor != birth.actor
                || breeding.cognition_policy != birth.cognition_policy
                || breeding.first_parent == breeding.second_parent
                || (lane.mode == HabitatMode::Wild
                    && !wild_birth_semantics_are_exact(
                        birth,
                        (!exact_historical_baseline)
                            .then_some(HEADLESS_WORLD_SIGNATURE_SCHEMA_VERSION),
                    ))
                || phenotype_mismatch
            {
                return Err(Ei0ExitGateError::Evidence(
                    "causal birth receipt does not match its resident genome",
                ));
            }
        }
    }
    if binding.adapter_name != REQUIRED_GPU_ADAPTER
        || binding.backend_api != REQUIRED_GPU_BACKEND_API
        || report.gpu_tests.len() != 2
        || report.gpu_tests.iter().any(|gpu| {
            gpu.adapter_name != REQUIRED_GPU_ADAPTER
                || gpu.backend_api != REQUIRED_GPU_BACKEND_API
                || gpu.adapter_name != binding.adapter_name
                || gpu.backend_api != binding.backend_api
        })
    {
        return Err(Ei0ExitGateError::Evidence(
            "GPU receipts do not match the locked adapter/API",
        ));
    }
    for gpu in &report.gpu_tests {
        let genome =
            genomes
                .get(&gpu.source_creature_genome_id.0)
                .ok_or(Ei0ExitGateError::Evidence(
                    "GPU source genome is absent from the population",
                ))?;
        let phenotype_matches = if let Some(genome) = genome.current() {
            expected_n2048_creature_phenotype_hash(genome)? == gpu.phenotype_hash
        } else {
            lifecycle
                .lanes
                .iter()
                .flat_map(|lane| &lane.births)
                .find(|birth| birth.genome_id == gpu.source_creature_genome_id)
                .is_some_and(|birth| birth.child_phenotype_hash == gpu.phenotype_hash)
        };
        if !phenotype_matches {
            return Err(Ei0ExitGateError::Evidence(
                "GPU phenotype does not match the bound genome evidence",
            ));
        }
    }
    if (!exact_historical_baseline
        && evaluate_clauses(lifecycle, &report.gpu_tests, baseline) != report.clauses)
        || !report.clauses.all_passed()
        || baseline.source_backend != "HeuristicBaseline"
        || baseline.promotion_eligible
        || baseline.hidden_promotion_trials != 0
        || !baseline.unknown_measures_preserved
        || baseline.unknown_measures.len() != 9
    {
        return Err(Ei0ExitGateError::Evidence(
            "committed clauses or heuristic boundary do not recompute",
        ));
    }
    Ok(())
}

fn is_exact_committed_historical_baseline(
    report: &Ei0ExitGateReport,
    binding: &Ei0ArtifactBinding,
    root: &Path,
) -> Result<bool, Ei0ExitGateError> {
    if report.schema_version != EI0_SCHEMA_3_HISTORICAL_VERSION
        || binding.producing_source_commit != EI0_SCHEMA_3_HISTORICAL_PRODUCER_COMMIT
    {
        return Ok(false);
    }

    let object = format!("HEAD:{COMMITTED_EI0_REPORT_PATH}");
    let committed_json = git_output(root, &["show", "--no-ext-diff", "--no-textconv", &object])?;
    let committed_report: Ei0ExitGateReport = serde_json::from_str(&committed_json)?;
    // Historical genomes retain raw JSON. Compare values so Git's Windows
    // line-ending conversion does not make an unchanged baseline look tampered.
    if serde_json::to_value(report)? != serde_json::to_value(&committed_report)? {
        return Err(Ei0ExitGateError::Evidence(
            "historical report differs from the committed baseline artifact",
        ));
    }
    Ok(true)
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_tools lives under <workspace>/crates")
        .to_path_buf()
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, Ei0ExitGateError> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| Ei0ExitGateError::Source(error.to_string()))?;
    if !output.status.success() {
        return Err(Ei0ExitGateError::Source(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn source_contract_digest_at_revision(
    root: &Path,
    revision: &str,
    paths: &[&str],
) -> Result<String, Ei0ExitGateError> {
    let mut hasher = blake3::Hasher::new();
    for path in paths {
        let object = format!("{revision}:{path}");
        let output = Command::new("git")
            .current_dir(root)
            .args(["show", "--no-ext-diff", "--no-textconv", &object])
            .output()
            .map_err(|error| Ei0ExitGateError::Source(error.to_string()))?;
        if !output.status.success() {
            return Err(Ei0ExitGateError::Source(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        let bytes = output.stdout;
        hasher.update(&(path.len() as u64).to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(&bytes);
    }
    Ok(format!("blake3-256:{}", hasher.finalize().to_hex()))
}

#[cfg(test)]
fn source_contract_digest(root: &Path, paths: &[&str]) -> Result<String, Ei0ExitGateError> {
    let revision = git_output(root, &["rev-parse", "HEAD"])?;
    source_contract_digest_at_revision(root, &revision, paths)
}

fn valid_blake3_text(value: &str) -> bool {
    value
        .strip_prefix("blake3-256:")
        .is_some_and(|hex| hex.len() == 64 && hex.chars().all(|ch| ch.is_ascii_hexdigit()))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("blake3-256:{}", blake3::hash(bytes).to_hex())
}

fn format_blake3(digest: Blake3Digest) -> String {
    let mut hex = String::with_capacity(64);
    for byte in digest.bytes() {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    format!("blake3-256:{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::SystemTime};

    fn run_git(root: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(root)
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    }

    #[test]
    fn source_contract_digest_uses_exact_git_blob_bytes_across_checkout_line_endings() {
        let nonce = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "alife-ei0-source-contract-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        run_git(&root, &["init", "--quiet"]);
        run_git(&root, &["config", "user.name", "EI0 Test"]);
        run_git(&root, &["config", "user.email", "ei0-test@example.invalid"]);

        let path = "contract.txt";
        let blob = b"alpha\nomega\n";
        fs::write(root.join(path), blob).unwrap();
        run_git(&root, &["add", "--", path]);
        run_git(&root, &["commit", "--quiet", "-m", "fixture"]);

        let mut expected_hasher = blake3::Hasher::new();
        expected_hasher.update(&(path.len() as u64).to_le_bytes());
        expected_hasher.update(path.as_bytes());
        expected_hasher.update(&(blob.len() as u64).to_le_bytes());
        expected_hasher.update(blob);
        let expected = format!("blake3-256:{}", expected_hasher.finalize().to_hex());
        let committed = source_contract_digest(&root, &[path]).unwrap();

        fs::write(root.join(path), b"alpha\r\nomega\r\n").unwrap();
        let crlf_checkout = source_contract_digest(&root, &[path]).unwrap();
        fs::remove_dir_all(&root).unwrap();

        assert_eq!(committed, expected);
        assert_eq!(crlf_checkout, expected);
    }
}
