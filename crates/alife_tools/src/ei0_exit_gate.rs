//! Reproducible Era 0 lifecycle and promotion-gate evidence.

use std::{collections::BTreeMap, path::Path, path::PathBuf};

use alife_archive::{CompositeGeneticArchiveInput, LineageLibrary, LineageLibraryConfig};
use alife_core::{
    ActiveChallengeResult, Blake3Digest, BrainCapacityClass, BrainScaleTier, CreatureGenome,
    FoundationGeneticIdentity, FoundationWeightAsset, GeneticLineageProvenance, GenomeId,
    HomeostaticSnapshot, LineageId, MemoryId, OrganismId, PhenotypeCompiler, PhenotypeHash,
    PolicyBackend, ScaffoldContractError, SensorProfile, Tick, Validate, Vec3f,
    ACTIVE_CHALLENGE_COUNT,
};
use alife_game_app::{
    CompositePopulationBirthReceipt, CompositePopulationRuntime, CompositePopulationRuntimeError,
    LifetimeInheritanceEvidence, MINIMUM_POST_RESTORE_TICKS,
};
use alife_gpu_backend::closed_loop_shader_bundle_digest;
use alife_training::{ActiveBatteryEvidence, N2048ActiveBatteryRunner};
use alife_world::{
    persist_composite_genetic_birth_assets, persist_creature_lifetime_state_asset, AssetManifest,
    AssetManifestEntry, CompositeGeneticSaveRef, CreatureAppearanceGenome,
    CreatureLifetimeMemoryRecord, CreatureLifetimeStateAsset, CreatureLifetimeStateSaveRef,
    CreatureLifetimeWeightValue, CreatureMindSaveSummary, CreatureSaveState, Habitat, HabitatActor,
    HabitatAuthority, HabitatAuthorityError, HabitatBreedingKind, HabitatBreedingRequest,
    HabitatId, HabitatMode, HeadlessScenarioBuilder, LearningTraceSaveSummary, PersistenceError,
    PortableSaveFile, RuntimeConfig, WeightLayerSaveSummary, P34_ASSET_MANIFEST_SCHEMA,
    P34_ASSET_MANIFEST_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};

pub const EI0_EXIT_GATE_SCHEMA_VERSION: u16 = 2;
const WORLD_SEED: u64 = 0xE10_0A11;
const WILD_HABITAT_RAW: u64 = 11;
const MANAGED_HABITAT_RAW: u64 = 12;
const FOUNDATION_ID_RAW: u64 = 0x4E32_3034_385F_5631;
const FOUNDATION_FAMILY_RAW: u64 = 0x4E32_3034_385F_FA11;
const SOURCE_RUN_ID: &str = "ei0-exit-gate-v2";

#[derive(Debug, thiserror::Error)]
pub enum Ei0ExitGateError {
    #[error("core contract failed: {0}")]
    Core(#[from] ScaffoldContractError),
    #[error("habitat authority failed: {0}")]
    Habitat(#[from] HabitatAuthorityError),
    #[error("portable save failed: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("restored population runtime failed: {0}")]
    PopulationRuntime(#[from] CompositePopulationRuntimeError),
    #[error("lineage archive failed: {0}")]
    Archive(#[from] alife_archive::ArchiveError),
    #[error("GPU active battery failed: {0}")]
    Training(#[from] alife_training::TrainingError),
    #[error("gate report I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("gate report JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("gate evidence is inconsistent: {0}")]
    Evidence(&'static str),
    #[error("gate execution failed after writing partial evidence: {0}")]
    Operational(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0LifetimeEvidence {
    pub memory_records: u32,
    pub lifetime_weights: u32,
    pub state_digest: String,
}

impl From<&LifetimeInheritanceEvidence> for Ei0LifetimeEvidence {
    fn from(value: &LifetimeInheritanceEvidence) -> Self {
        Self {
            memory_records: value.memory_records,
            lifetime_weights: value.lifetime_weights,
            state_digest: value.state_digest.clone(),
        }
    }
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
    pub cognition_policy: PolicyBackend,
    pub post_restore_ticks: u32,
    pub first_parent_lifetime: Ei0LifetimeEvidence,
    pub second_parent_lifetime: Ei0LifetimeEvidence,
    pub child_lifetime: Ei0LifetimeEvidence,
    pub gpu_intent_sequence_id: Option<u64>,
    pub gpu_intent_world_tick: Option<Tick>,
    pub gpu_selected_mate: Option<OrganismId>,
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
    pub archive_birth_manifest_count: u64,
    pub lineage_compare_passed: bool,
    pub no_lifetime_state_inherited: bool,
    pub player_directed_wild_breeding_rejected: bool,
    pub creature_directed_managed_breeding_rejected: bool,
    pub lanes: Vec<Ei0LaneReceipt>,
    pub population_genomes: Vec<CreatureGenome>,
    pub evidence_digests: Ei0EvidenceDigests,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ei0LifecycleEvidence {
    pub report: Ei0LifecycleGateReport,
    pub final_generation_genomes: Vec<CreatureGenome>,
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

    fn unavailable(detail: &str) -> Self {
        let unavailable = || Ei0ClauseEvidence {
            status: Ei0EvidenceStatus::Unavailable,
            detail: detail.to_string(),
        };
        Self {
            run: unavailable(),
            observe: unavailable(),
            save_load: unavailable(),
            wild_breed: unavailable(),
            managed_breed: unavailable(),
            test: unavailable(),
            archive: unavailable(),
            compare: unavailable(),
            stable_multi_generation_population: unavailable(),
            gpu_policy_identity: unavailable(),
            no_hidden_policy_control: unavailable(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ei0ExitVerdict {
    pub era0_exit_gate_passed: bool,
    pub era1_promotion_evaluated: bool,
    pub era1_status: String,
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
    pub operational_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ei0GateExecution {
    pub report: Ei0ExitGateReport,
    pub operational_error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BaselineFixture {
    source_backends: Vec<String>,
    layer_counts: BaselineLayerCounts,
    evidence_scope: BaselineEvidenceScope,
    measures: BTreeMap<String, BaselineReading>,
    objectives: BTreeMap<String, BaselineReading>,
    promotion_eligible: bool,
}

#[derive(Debug, Deserialize)]
struct BaselineLayerCounts {
    hidden_promotion: u64,
}

#[derive(Debug, Deserialize)]
struct BaselineEvidenceScope {
    promotion_backend_eligible: bool,
    unsupported_measures: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BaselineReading {
    value: Option<f64>,
    samples: u64,
}

struct FounderSaveReceipt {
    save_path: PathBuf,
    wild_id: HabitatId,
    managed_id: HabitatId,
    wild_founders: [OrganismId; 4],
    managed_founders: [OrganismId; 4],
    source_genome_digests: BTreeMap<String, String>,
    foundation_digest: String,
    save_digest: String,
}

pub fn run_ei0_lifecycle_gate(
    evidence_root: impl AsRef<Path>,
) -> Result<Ei0LifecycleEvidence, Ei0ExitGateError> {
    let evidence_root = evidence_root.as_ref();
    std::fs::create_dir_all(evidence_root)?;
    let founder_save = write_founder_population_save(evidence_root)?;
    let restored = PortableSaveFile::from_json_file(&founder_save.save_path)?;
    restored.validate_with_asset_root(evidence_root)?;
    let mut tampered_save = restored.clone();
    tampered_save.deterministic_seed = tampered_save.deterministic_seed.wrapping_add(1);
    let tampered_save_rejected = tampered_save
        .validate_with_asset_root(evidence_root)
        .is_err();
    drop(restored);

    // From this point forward, every genome and brain asset is loaded solely
    // through the portable save. No pre-save CreatureGenome remains in scope.
    let mut population =
        CompositePopulationRuntime::restore_from_file(&founder_save.save_path, evidence_root)?;
    let restored_population_count = population.residents().count();
    let run_observed = observe_restored_population(&population)?;
    population.advance_ticks(MINIMUM_POST_RESTORE_TICKS);

    let player_directed_wild_breeding_rejected = population
        .apply_player_breed_command(
            founder_save.wild_id,
            founder_save.wild_founders[0],
            founder_save.wild_founders[1],
            OrganismId(8_001),
            0xE10_8001,
        )
        .is_err();
    let creature_directed_managed_breeding_rejected = population
        .world_snapshot()
        .habitat_authority()
        .authorize_breeding(HabitatBreedingRequest {
            habitat_id: founder_save.managed_id,
            first_parent: founder_save.managed_founders[0],
            second_parent: founder_save.managed_founders[1],
            kind: HabitatBreedingKind::CreatureChosen,
            actor: HabitatActor::Organism(founder_save.managed_founders[0]),
            tick: population.world_snapshot().tick(),
        })
        .is_err();

    let mut reproduction_runner = N2048ActiveBatteryRunner::new_required()?;
    let wild_first = execute_wild_birth(
        &mut population,
        &mut reproduction_runner,
        founder_save.wild_id,
        founder_save.wild_founders[0],
        OrganismId(3_001),
        0xE10_3001,
    )?;
    let wild_second = execute_wild_birth(
        &mut population,
        &mut reproduction_runner,
        founder_save.wild_id,
        founder_save.wild_founders[2],
        OrganismId(3_002),
        0xE10_3002,
    )?;
    let wild_final = execute_wild_birth(
        &mut population,
        &mut reproduction_runner,
        founder_save.wild_id,
        OrganismId(3_001),
        OrganismId(3_003),
        0xE10_3003,
    )?;

    let managed_first = execute_managed_birth(
        &mut population,
        founder_save.managed_id,
        founder_save.managed_founders[0],
        founder_save.managed_founders[1],
        OrganismId(4_001),
        0xE10_4001,
    )?;
    let managed_second = execute_managed_birth(
        &mut population,
        founder_save.managed_id,
        founder_save.managed_founders[2],
        founder_save.managed_founders[3],
        OrganismId(4_002),
        0xE10_4002,
    )?;
    let managed_final = execute_managed_birth(
        &mut population,
        founder_save.managed_id,
        OrganismId(4_001),
        OrganismId(4_002),
        OrganismId(4_003),
        0xE10_4003,
    )?;

    let lanes = vec![
        Ei0LaneReceipt {
            mode: HabitatMode::Wild,
            habitat_id: founder_save.wild_id,
            births: vec![wild_first, wild_second, wild_final],
        },
        Ei0LaneReceipt {
            mode: HabitatMode::Managed,
            habitat_id: founder_save.managed_id,
            births: vec![managed_first, managed_second, managed_final],
        },
    ];
    let final_generation_genomes = [OrganismId(3_003), OrganismId(4_003)]
        .map(|organism_id| {
            population
                .resident(organism_id)
                .map(|resident| resident.genome.clone())
                .ok_or(Ei0ExitGateError::Evidence(
                    "runtime is missing its final generation",
                ))
        })
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    let archive_root = evidence_root.join("lineage-library");
    let mut library = LineageLibrary::open(LineageLibraryConfig::profile_default(&archive_root))?;
    let mut archive_manifests = BTreeMap::new();
    let mut archive_composite_assets = BTreeMap::new();
    let mut source_genomes = founder_save.source_genome_digests;
    let mut lineage_compare_passed = true;
    let world = population.world_snapshot();
    for resident in population.residents() {
        source_genomes.insert(
            resident.organism_id.raw().to_string(),
            digest_bytes(&serde_json::to_vec(&resident.genome)?),
        );
        let expressed = resident.genome.express()?;
        let development = expressed.development_state_at(Tick::new(u64::from(
            expressed.development.maturation_duration_ticks,
        )))?;
        let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &expressed.brain_genome,
            &BrainCapacityClass::production_for_id(resident.genome.foundation.brain_class_id)?,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &resident.foundation,
        )?;
        if phenotype.phenotype_hash() != resident.phenotype_hash {
            return Err(Ei0ExitGateError::Evidence(
                "runtime phenotype changed before archive",
            ));
        }
        let birth_tick = world
            .habitat_authority()
            .membership(resident.organism_id)
            .ok_or(Ei0ExitGateError::Evidence(
                "archived resident is missing habitat membership",
            ))?
            .entered_tick;
        let foundation_bytes = resident.foundation.encode_canonical()?;
        let manifest_digest = library.archive_composite_birth(CompositeGeneticArchiveInput {
            source_run_id: SOURCE_RUN_ID,
            organism_id: resident.organism_id,
            birth_tick,
            creature_genome: &resident.genome,
            phenotype: &phenotype,
            foundation_asset_bytes: &foundation_bytes,
        })?;
        let manifest = library.load_manifest(manifest_digest)?;
        let archived = library.load_creature_genome(&manifest)?;
        lineage_compare_passed &= archived == resident.genome;
        archive_manifests.insert(
            resident.organism_id.raw().to_string(),
            format_blake3(manifest_digest),
        );
        let composite =
            manifest
                .genetic
                .composite_genome_asset
                .as_ref()
                .ok_or(Ei0ExitGateError::Evidence(
                    "archive manifest is missing composite genome asset",
                ))?;
        archive_composite_assets.insert(
            resident.organism_id.raw().to_string(),
            format_blake3(composite.digest),
        );
    }
    let archive_birth_manifest_count = library.manifest_count()?;
    let live_population_count = population.residents().count();
    let population_genomes = population
        .residents()
        .map(|resident| resident.genome.clone())
        .collect::<Vec<_>>();
    let no_lifetime_state_inherited = lanes.iter().all(lane_proves_noninheritance)
        && population
            .residents()
            .filter(|resident| !resident.restored_from_save)
            .all(|resident| {
                resident.lifetime_state.memory_records.is_empty()
                    && resident.lifetime_state.lifetime_weight_values.is_empty()
            });
    let tampered_provenance_rejected = final_generation_genomes.first().is_some_and(|genome| {
        let mut tampered = genome.clone();
        tampered.provenance.conception_seed = tampered.provenance.conception_seed.wrapping_add(1);
        tampered.validate_contract().is_err()
    });
    let evidence_digests = Ei0EvidenceDigests {
        source_genomes,
        foundation_weights: Some(founder_save.foundation_digest),
        shader_bundle: Some(format_blake3(closed_loop_shader_bundle_digest())),
        portable_save: Some(founder_save.save_digest),
        archive_manifests,
        archive_composite_assets,
    };
    let report = Ei0LifecycleGateReport {
        schema_version: EI0_EXIT_GATE_SCHEMA_VERSION,
        founder_count: 8,
        live_population_count,
        generation_count: 3,
        run_observed,
        portable_save_round_trip: restored_population_count == 8,
        tampered_save_rejected,
        tampered_provenance_rejected,
        restored_population_count,
        post_restore_ticks: population.post_restore_ticks(),
        archive_birth_manifest_count,
        lineage_compare_passed,
        no_lifetime_state_inherited,
        player_directed_wild_breeding_rejected,
        creature_directed_managed_breeding_rejected,
        lanes,
        population_genomes,
        evidence_digests,
    };
    Ok(Ei0LifecycleEvidence {
        report,
        final_generation_genomes,
    })
}

fn write_founder_population_save(
    evidence_root: &Path,
) -> Result<FounderSaveReceipt, Ei0ExitGateError> {
    let wild_id = HabitatId::new(WILD_HABITAT_RAW).ok_or(Ei0ExitGateError::Evidence(
        "wild habitat id must be nonzero",
    ))?;
    let managed_id = HabitatId::new(MANAGED_HABITAT_RAW).ok_or(Ei0ExitGateError::Evidence(
        "managed habitat id must be nonzero",
    ))?;
    let wild_founders = [
        OrganismId(1_001),
        OrganismId(1_002),
        OrganismId(1_003),
        OrganismId(1_004),
    ];
    let managed_founders = [
        OrganismId(2_001),
        OrganismId(2_002),
        OrganismId(2_003),
        OrganismId(2_004),
    ];
    let mut authority = HabitatAuthority::new(vec![
        Habitat::new(wild_id, "Era 0 Wild", HabitatMode::Wild)?,
        Habitat::new(managed_id, "Era 0 Managed", HabitatMode::Managed)?,
    ])?;
    for organism_id in wild_founders {
        authority.register_creature(organism_id, wild_id, Tick::ZERO)?;
    }
    for organism_id in managed_founders {
        authority.register_creature(organism_id, managed_id, Tick::ZERO)?;
    }
    let mut world = HeadlessScenarioBuilder::new(WORLD_SEED)
        .agent("wild-a", wild_founders[0], Vec3f::ZERO)
        .social_agent("wild-b", wild_founders[1], Vec3f::new(0.5, 0.0, 0.0), 0.9)
        .social_agent("wild-c", wild_founders[2], Vec3f::new(20.0, 0.0, 0.0), 0.9)
        .social_agent("wild-d", wild_founders[3], Vec3f::new(20.5, 0.0, 0.0), 0.9)
        .social_agent(
            "managed-a",
            managed_founders[0],
            Vec3f::new(0.0, 40.0, 0.0),
            0.9,
        )
        .social_agent(
            "managed-b",
            managed_founders[1],
            Vec3f::new(0.5, 40.0, 0.0),
            0.9,
        )
        .social_agent(
            "managed-c",
            managed_founders[2],
            Vec3f::new(20.0, 40.0, 0.0),
            0.9,
        )
        .social_agent(
            "managed-d",
            managed_founders[3],
            Vec3f::new(20.5, 40.0, 0.0),
            0.9,
        )
        .food("era0-food", Vec3f::new(100.0, 100.0, 0.0), 0.8)
        .hazard("era0-hazard", Vec3f::new(-100.0, -100.0, 0.0), 0.4)
        .build()?;
    world.replace_habitat_authority(authority)?;

    let foundation = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let foundation_digest = format_blake3(foundation.digest());
    let mut entries = Vec::new();
    let mut creatures = Vec::new();
    let mut source_genome_digests = BTreeMap::new();
    for (index, organism_id) in wild_founders
        .into_iter()
        .chain(managed_founders)
        .enumerate()
    {
        let genome = founder_genome(WORLD_SEED + index as u64 + 1)?;
        let expressed = genome.express()?;
        let development = expressed.development_state_at(Tick::new(u64::from(
            expressed.development.maturation_duration_ticks,
        )))?;
        let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &expressed.brain_genome,
            &BrainCapacityClass::n2048(),
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &foundation,
        )?;
        let (composite, composite_entries) = persist_composite_genetic_birth_assets(
            evidence_root,
            &genome,
            &foundation,
            phenotype.phenotype_hash(),
        )?;
        extend_unique_assets(&mut entries, composite_entries);
        let lifetime = CreatureLifetimeStateAsset {
            schema_version: 1,
            organism_id,
            memory_records: vec![CreatureLifetimeMemoryRecord {
                memory_id: MemoryId(10_001 + index as u64),
                source_organism_id: organism_id,
                value_q16: 30_001 + index as u32,
            }],
            lifetime_weight_values: vec![CreatureLifetimeWeightValue {
                synapse_index: 100 + index as u32,
                value: 0.0625 * (index as f32 + 1.0),
            }],
        };
        let (lifetime_ref, lifetime_entry) =
            persist_creature_lifetime_state_asset(evidence_root, &lifetime)?;
        extend_unique_assets(&mut entries, [lifetime_entry]);
        source_genome_digests.insert(
            organism_id.raw().to_string(),
            digest_bytes(&serde_json::to_vec(&genome)?),
        );
        creatures.push(founder_creature_save(
            organism_id,
            &genome,
            composite,
            lifetime_ref,
            &lifetime,
        ));
    }
    let save = PortableSaveFile::from_headless_world(
        "ei0-restored-founder-population",
        &world,
        RuntimeConfig::deterministic_default(WORLD_SEED, BrainScaleTier::Standard2048),
        AssetManifest {
            schema: P34_ASSET_MANIFEST_SCHEMA.to_string(),
            schema_version: P34_ASSET_MANIFEST_SCHEMA_VERSION,
            entries,
        },
        creatures,
    )?;
    let save_path = evidence_root.join("ei0-founder-population.alife.json");
    save.to_json_file(&save_path)?;
    let save_digest = digest_bytes(&std::fs::read(&save_path)?);
    Ok(FounderSaveReceipt {
        save_path,
        wild_id,
        managed_id,
        wild_founders,
        managed_founders,
        source_genome_digests,
        foundation_digest,
        save_digest,
    })
}

fn extend_unique_assets(
    entries: &mut Vec<AssetManifestEntry>,
    additions: impl IntoIterator<Item = AssetManifestEntry>,
) {
    for entry in additions {
        if !entries
            .iter()
            .any(|present| present.asset_id == entry.asset_id)
        {
            entries.push(entry);
        }
    }
}

fn founder_creature_save(
    organism_id: OrganismId,
    genome: &CreatureGenome,
    composite_genetics: CompositeGeneticSaveRef,
    lifetime_state_asset: CreatureLifetimeStateSaveRef,
    lifetime: &CreatureLifetimeStateAsset,
) -> CreatureSaveState {
    CreatureSaveState {
        organism_id,
        genome_id: genome.id,
        brain_class: BrainScaleTier::Standard2048,
        development_tick: Tick::ZERO,
        appearance: CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick: Tick::ZERO,
            homeostasis: HomeostaticSnapshot::baseline(Tick::ZERO),
            memory_record_count: lifetime.memory_records.len() as u32,
            memory_source_ids: lifetime
                .memory_records
                .iter()
                .map(|record| record.memory_id)
                .collect(),
            concept_count: 0,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".to_string(),
            diagnostics: vec!["Era 0 restored founder".to_string()],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest: format!("fnv1a64:{:016x}", genome.id.0),
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: lifetime.lifetime_weight_values.len() as u32,
            h_operational_entries: 1,
            h_shadow_entries: 0,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: true,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: Some(Tick::ZERO),
        },
        composite_genetics: Some(composite_genetics),
        lifetime_state_asset: Some(lifetime_state_asset),
        gpu_brain: None,
    }
}

fn founder_genome(seed: u64) -> Result<CreatureGenome, ScaffoldContractError> {
    CreatureGenome::early_mammal_founder(
        seed,
        FoundationGeneticIdentity::new(
            FOUNDATION_ID_RAW,
            1,
            FOUNDATION_FAMILY_RAW,
            BrainCapacityClass::N2048_ID,
        )?,
    )
}

fn observe_restored_population(
    population: &CompositePopulationRuntime,
) -> Result<bool, ScaffoldContractError> {
    let mut world = population.world_snapshot();
    let tick = world.tick();
    for resident in population.residents() {
        let frame = world.perception_frame(
            resident.organism_id,
            tick,
            SensorProfile::GroundedObjectSlotsV1,
            HomeostaticSnapshot::baseline(tick),
        )?;
        if frame.candidates().is_empty() {
            return Ok(false);
        }
    }
    Ok(true)
}

fn execute_wild_birth(
    population: &mut CompositePopulationRuntime,
    runner: &mut N2048ActiveBatteryRunner,
    habitat_id: HabitatId,
    initiator: OrganismId,
    child: OrganismId,
    conception_seed: u64,
) -> Result<Ei0BirthReceipt, Ei0ExitGateError> {
    let genome = population
        .resident(initiator)
        .ok_or(Ei0ExitGateError::Evidence("wild initiator is absent"))?
        .genome
        .clone();
    let mut observed_world = population.world_snapshot();
    let intent = runner.run_creature_chosen_reproduction_intent_in_world(
        initiator,
        &genome,
        &mut observed_world,
        256,
    )?;
    let birth = population.apply_gpu_reproduction_intent(
        habitat_id,
        observed_world,
        &intent.patch,
        child,
        conception_seed,
    )?;
    birth_receipt(population, birth, Some(&intent))
}

fn execute_managed_birth(
    population: &mut CompositePopulationRuntime,
    habitat_id: HabitatId,
    first_parent: OrganismId,
    second_parent: OrganismId,
    child: OrganismId,
    conception_seed: u64,
) -> Result<Ei0BirthReceipt, Ei0ExitGateError> {
    let birth = population.apply_player_breed_command(
        habitat_id,
        first_parent,
        second_parent,
        child,
        conception_seed,
    )?;
    birth_receipt(population, birth, None)
}

fn birth_receipt(
    population: &CompositePopulationRuntime,
    birth: CompositePopulationBirthReceipt,
    intent: Option<&alife_training::GpuReproductionIntentReceipt>,
) -> Result<Ei0BirthReceipt, Ei0ExitGateError> {
    let genome = &population
        .resident(birth.child_organism_id)
        .ok_or(Ei0ExitGateError::Evidence("birth child is absent"))?
        .genome;
    Ok(Ei0BirthReceipt {
        organism_id: birth.child_organism_id,
        genome_id: birth.child_genome_id,
        lineage_id: genome.lineage_id,
        parent_genome_ids: genome.parent_genome_ids.clone(),
        generation: birth.child_generation,
        conception_seed: genome.conception_seed,
        ordinary_birth: genome.provenance.ordinary_birth,
        provenance: genome.provenance.clone(),
        foundation_id: genome.foundation.foundation_id,
        foundation_version: genome.foundation.version,
        compatibility_family_id: genome.foundation.compatibility_family_id,
        breeding_kind: birth.breeding.kind,
        actor: birth.breeding.actor,
        cognition_policy: birth.breeding.cognition_policy,
        post_restore_ticks: birth.post_restore_ticks,
        first_parent_lifetime: (&birth.first_parent_lifetime).into(),
        second_parent_lifetime: (&birth.second_parent_lifetime).into(),
        child_lifetime: (&birth.child_lifetime).into(),
        gpu_intent_sequence_id: intent.map(|receipt| receipt.patch.header().sequence_id.0),
        gpu_intent_world_tick: intent.map(|receipt| receipt.patch.header().world_tick),
        gpu_selected_mate: intent.map(|receipt| receipt.mate_organism_id),
    })
}

fn lane_proves_noninheritance(lane: &Ei0LaneReceipt) -> bool {
    lane.births.len() == 3
        && lane.births.iter().all(|birth| {
            birth.child_lifetime.memory_records == 0 && birth.child_lifetime.lifetime_weights == 0
        })
        && lane.births[..2].iter().all(|birth| {
            birth.first_parent_lifetime.memory_records > 0
                && birth.first_parent_lifetime.lifetime_weights > 0
                && birth.second_parent_lifetime.memory_records > 0
                && birth.second_parent_lifetime.lifetime_weights > 0
                && birth.first_parent_lifetime.state_digest
                    != birth.second_parent_lifetime.state_digest
        })
}

pub fn run_ei0_exit_gate(
    evidence_root: impl AsRef<Path>,
) -> Result<Ei0ExitGateReport, Ei0ExitGateError> {
    let evidence_root = evidence_root.as_ref();
    let lifecycle_evidence = run_ei0_lifecycle_gate(evidence_root.join("lifecycle"))?;
    let mut runner = N2048ActiveBatteryRunner::new_required()?;
    let mut gpu_tests = Vec::with_capacity(lifecycle_evidence.final_generation_genomes.len());
    for (lane, genome) in lifecycle_evidence
        .report
        .lanes
        .iter()
        .zip(&lifecycle_evidence.final_generation_genomes)
    {
        let final_birth = lane.births.last().ok_or(Ei0ExitGateError::Evidence(
            "lane is missing its final birth",
        ))?;
        if final_birth.genome_id != genome.id {
            return Err(Ei0ExitGateError::Evidence(
                "lane final birth does not match the GPU test genome",
            ));
        }
        gpu_tests.push(gpu_receipt(
            runner.run_creature_genome(final_birth.organism_id, genome)?,
        )?);
    }
    let heuristic_baseline = load_heuristic_baseline_boundary()?;
    let lifecycle = lifecycle_evidence.report;
    let evidence_digests = lifecycle.evidence_digests.clone();
    let clauses = evaluate_clauses(&lifecycle, &gpu_tests, &heuristic_baseline);
    let verdict = Ei0ExitVerdict {
        era0_exit_gate_passed: clauses.all_passed(),
        era1_promotion_evaluated: false,
        era1_status: "OUT_OF_SCOPE".to_string(),
    };
    Ok(Ei0ExitGateReport {
        schema_version: EI0_EXIT_GATE_SCHEMA_VERSION,
        verdict,
        clauses,
        lifecycle: Some(lifecycle),
        gpu_tests,
        heuristic_baseline: Some(heuristic_baseline),
        evidence_digests,
        operational_error: None,
    })
}

pub fn execute_ei0_exit_gate(evidence_root: impl AsRef<Path>) -> Ei0GateExecution {
    match run_ei0_exit_gate(evidence_root) {
        Ok(report) => Ei0GateExecution {
            report,
            operational_error: None,
        },
        Err(error) => {
            let message = error.to_string();
            Ei0GateExecution {
                report: partial_report(&message),
                operational_error: Some(message),
            }
        }
    }
}

pub fn run_ei0_exit_gate_and_write(
    evidence_root: impl AsRef<Path>,
    output: impl AsRef<Path>,
) -> Result<Ei0ExitGateReport, Ei0ExitGateError> {
    let execution = execute_ei0_exit_gate(evidence_root);
    write_ei0_exit_gate_report(output, &execution.report)?;
    if let Some(error) = execution.operational_error {
        Err(Ei0ExitGateError::Operational(error))
    } else if !execution.report.verdict.era0_exit_gate_passed {
        Err(Ei0ExitGateError::Operational(
            "Era 0 exit gate failed; inspect the emitted report".to_string(),
        ))
    } else {
        Ok(execution.report)
    }
}

fn partial_report(error: &str) -> Ei0ExitGateReport {
    Ei0ExitGateReport {
        schema_version: EI0_EXIT_GATE_SCHEMA_VERSION,
        verdict: Ei0ExitVerdict {
            era0_exit_gate_passed: false,
            era1_promotion_evaluated: false,
            era1_status: "OUT_OF_SCOPE".to_string(),
        },
        clauses: Ei0ExitClauses::unavailable(error),
        lifecycle: None,
        gpu_tests: Vec::new(),
        heuristic_baseline: None,
        evidence_digests: Ei0EvidenceDigests::default(),
        operational_error: Some(error.to_string()),
    }
}

pub fn write_ei0_exit_gate_report(
    path: impl AsRef<Path>,
    report: &Ei0ExitGateReport,
) -> Result<(), Ei0ExitGateError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(report)?)?;
    Ok(())
}

fn gpu_receipt(evidence: ActiveBatteryEvidence) -> Result<Ei0GpuBatteryReceipt, Ei0ExitGateError> {
    let source_creature_genome_id =
        evidence
            .source_creature_genome_id
            .ok_or(Ei0ExitGateError::Evidence(
                "GPU receipt is missing the source creature genome",
            ))?;
    let lineage_id = evidence.lineage_id.ok_or(Ei0ExitGateError::Evidence(
        "GPU receipt is missing lineage identity",
    ))?;
    Ok(Ei0GpuBatteryReceipt {
        organism_id: evidence.receipt.organism_id,
        source_creature_genome_id,
        brain_genome_id: evidence.brain_genome_id,
        parent_genome_ids: evidence.parent_genome_ids,
        lineage_id,
        phenotype_hash: evidence.phenotype_hash,
        foundation_id: evidence.foundation_id,
        foundation_version: evidence.foundation_version,
        compatibility_family_id: evidence.compatibility_family_id,
        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
        completed_challenges: evidence.receipt.completed_count(),
        challenge_results: evidence.receipt.results,
        challenge_worlds: evidence.challenge_worlds,
        gpu_dispatches: evidence.gpu_dispatches,
        sealed_outcomes: evidence.sealed_outcomes,
        sleep_consolidations: evidence.sleep_consolidations,
        slm_enabled: evidence.slm_enabled,
        adapter_name: evidence.adapter_name,
        backend_api: evidence.backend_api,
    })
}

fn load_heuristic_baseline_boundary() -> Result<Ei0HeuristicBaselineBoundary, Ei0ExitGateError> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("reports")
        .join("ei0_real_fixture_report.json");
    let fixture: BaselineFixture = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let unknown_measures = fixture.evidence_scope.unsupported_measures.clone();
    let unknown_measures_preserved = unknown_measures.len() == 9
        && unknown_measures.iter().all(|name| {
            let reading = match name.as_str() {
                "cognitive_objective" => fixture.objectives.get("cognitive"),
                "social_objective" => fixture.objectives.get("social"),
                "group_objective" => fixture.objectives.get("group"),
                other => fixture.measures.get(other),
            };
            reading.is_some_and(|reading| reading.value.is_none() && reading.samples == 0)
        });
    Ok(Ei0HeuristicBaselineBoundary {
        source_backend: fixture.source_backends.join(","),
        promotion_eligible: fixture.promotion_eligible,
        hidden_promotion_trials: fixture.layer_counts.hidden_promotion,
        unknown_measures,
        unknown_measures_preserved: unknown_measures_preserved
            && !fixture.evidence_scope.promotion_backend_eligible,
    })
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
            lane.births.len() == 3
                && lane.births.iter().all(|birth| {
                    birth.breeding_kind == HabitatBreedingKind::CreatureChosen
                        && matches!(birth.actor, HabitatActor::Organism(_))
                        && birth.cognition_policy == PolicyBackend::NeuralClosedLoopGpu
                        && birth.gpu_intent_sequence_id.is_some()
                        && birth.gpu_selected_mate.is_some()
                })
        });
    let managed_breed = lifecycle.creature_directed_managed_breeding_rejected
        && managed_lane.is_some_and(|lane| {
            lane.births.len() == 3
                && lane.births.iter().all(|birth| {
                    birth.breeding_kind == HabitatBreedingKind::Explicit
                        && birth.actor == HabitatActor::Player
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
                && !receipt.adapter_name.trim().is_empty()
                && !receipt.backend_api.trim().is_empty()
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
                && lifecycle.population_genomes.len() == 14,
            "runtime births retain genetics and reset nonzero parental lifetime state",
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

pub fn default_report_path(root: impl AsRef<Path>) -> PathBuf {
    root.as_ref().join("ei0_exit_gate_report.json")
}
